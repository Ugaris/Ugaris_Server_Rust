//! Two-City thieves-guild master (`CDR_TWOTHIEFMASTER`) - "Guild Master",
//! the lockpick-chain quest giver behind `world::npc::area17::thiefguard`'s
//! sewer entrance.
//!
//! Ports `src/area/17/two.c::thiefmaster` (`:1746-2188`) plus its death
//! hook `thiefmaster_dead` (`:2190-2209`, in `ugaris-server::world_events::
//! death_hooks`, since it needs `PlayerRuntime`).
//!
//! Runs the 18-state (`4` through `21`) "Earning the Lockpick" chain: four
//! quests (25 "Earning the Lockpick", 26 "Extortion", 27 "Price Fix
//! Exposed", 28 "The Golden Lockpick"), each gated on `has_item` checks
//! for the previous reward (a real `lockpick`/`sewer_key1`/`sewer_key2`)
//! that let a returning player silently skip ahead to wherever their
//! inventory says they already are - and, symmetrically, silently fall
//! back to an earlier state (with a scolding line) if a prerequisite item
//! was lost. Quests 25/26 complete via an `NT_CHAR` state (`10`/`14`)
//! whose reward tier depends on `twocity_ppd.thief_killed[6]` (fed by the
//! already-ported `PlayerRuntime::mark_twocity_burndown_kill`/
//! `ugaris-server::world_events::death_hooks::
//! apply_two_robber_death_from_hurt_event`); quests 27/28 complete via
//! `NT_GIVE` (a merchant's note/a golden lockpick).
//!
//! Quest 25 additionally needs `NT_TEXT`'s `"i am done"` (a new
//! `TWOCITY_QA` row, answer_code `16`) to transition `thief_state` `9`
//! (waiting) -> `10` (ready to grade).
//!
//! The exp reward for quests 25/26 is computed and granted directly
//! inside `World` (`World::give_exp`, `crate::quest::scale_exp`,
//! `level_value`) since it only needs `Character::level` and the prior
//! completion count - the latter is supplied as a per-player fact
//! (`TwoThiefMasterPlayerFacts::quest25_count`/`quest26_count`, C's own
//! `questlog_count(co, N)` called *before* the `questlog_done` increment)
//! rather than deferred like `world::npc::area17::alchemist`'s reward,
//! since `World` cannot otherwise reach `PlayerRuntime::quest_log`. Only
//! the resulting bookkeeping (`thief_killed` reset, `thief_bits`,
//! `quest_log.mark_done`/`sendquestlog`, and the new item, all
//! `PlayerRuntime`/`ZoneLoader`-only) is deferred via
//! [`TwoThiefMasterOutcomeEvent::Quest25Reward`]/[`TwoThiefMasterOutcomeEvent::Quest26Reward`].
//! Quests 27/28 use real `QUEST_TABLE` exp values, so their completion
//! follows `alchemist`'s/`two_skelly`'s precedent instead: `World` says
//! the reward line directly (`NT_GIVE` already knows the giver's name),
//! and defers only `quest_log.complete_legacy`/the item to
//! [`TwoThiefMasterOutcomeEvent::Quest27Done`]/[`TwoThiefMasterOutcomeEvent::Quest28Done`].
//!
//! Every `questlog_open`/`questlog_close` call along the chain (four
//! distinct quest numbers, scattered across states `5`/`11`/`15`/`18`) is
//! ported through a pair of generic [`TwoThiefMasterOutcomeEvent::QuestOpen`]/
//! [`TwoThiefMasterOutcomeEvent::QuestClose`] events instead of one
//! dedicated variant per quest number, since C calls both with a plain
//! numeric argument at each site.
//!
//! Unlike `thiefguard`, this driver has no explicit `NT_GOTHIT` handler of
//! its own in C - its per-tick tail's `standard_message_driver(cn, msg, 0,
//! 0)` fallback call (`two.c:2162`) is the *only* source of hostility
//! (`agressive=0`/`helper=0` no-op the `NT_CHAR`/`NT_SEEHIT` cases, same
//! as `thiefguard`'s own module doc comment explains), so its `NT_GOTHIT`
//! case (`fight_driver_note_hit` plus a group/`can_attack`-gated
//! `fight_driver_add_enemy(cn, co, 1, ...)`, `drvlib.c:2512-2538`) is
//! ported here directly rather than reused from a shared helper (none
//! exists yet for a non-`CDR_SIMPLEBADDY` `driver_state`); the C
//! branch's `char_see_char`-based `visible` argument has no equivalent in
//! [`add_simple_baddy_enemy_unchecked`] (it always records a fresh enemy
//! as `visible: false`, same simplification `thiefguard`'s own enemy-add
//! call already relies on), so it is not computed here either. Also like
//! `thiefguard`, the per-tick tail calls `fight_driver_attack_visible(cn,
//! 0)` but never `fight_driver_follow_invisible` (`two.c:2169-2172`), so
//! `World::fight_driver_attack_visible_and_follow`'s `may_follow_
//! invisible: false` is used again here.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_TWOTHIEFMASTER};
use crate::drvlib::offset2dx;
use crate::item_driver::{
    IID_AREA17_GOLDENLOCKPICK, IID_AREA17_LOCKPICK, IID_AREA17_MERCHANTNOTE1,
    IID_AREA17_PALACEKEY3, IID_AREA17_SEWERKEY1, IID_AREA17_SEWERKEY2,
};
use crate::quest::scale_exp;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;

use super::TWOCITY_QA;

/// C `char_dist(cn, co) > 10` (`two.c:1795`).
const TWO_THIEFMASTER_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`two.c:1778`).
const TWO_THIEFMASTER_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`two.c:1783`, `:2067`).
const TWO_THIEFMASTER_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`two.c:2181`): idle "return to post" threshold.
const TWO_THIEFMASTER_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `realtime - ppd->thief_last_seen > 60 * 2` (`two.c:1850`): the
/// `thief_state == 9` waiting-for-mission nag cooldown, in wall-clock
/// seconds.
const TWO_THIEFMASTER_WAIT_NAG_SECONDS: i32 = 60 * 2;

/// C `struct thiefmaster_data` (`two.c:1741-1744`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoThiefMasterDriverData {
    pub last_talk_tick: u64,
    pub current_victim: Option<CharacterId>,
}

/// Per-player facts [`World::process_two_thiefmaster_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoThiefMasterPlayerFacts {
    pub thief_state: i32,
    pub thief_last_seen: i32,
    pub thief_killed: [i32; 6],
    /// C `questlog_count(co, 25)`, read *before* the completion increment
    /// (`quest[25].done`), used to scale the quest-25 exp reward.
    pub quest25_count: u8,
    /// Same, for quest 26.
    pub quest26_count: u8,
}

/// A side effect [`World::process_two_thiefmaster_actions`] could not
/// apply directly because it touches `PlayerRuntime`/needs `ZoneLoader`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoThiefMasterOutcomeEvent {
    /// Write the new `twocity_ppd.thief_state` back.
    UpdateThiefState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->thief_last_seen = realtime;` (`two.c:2058`).
    UpdateThiefLastSeen {
        player_id: CharacterId,
        realtime: i32,
    },
    /// C `questlog_open(co, quest)`.
    QuestOpen { player_id: CharacterId, quest: u8 },
    /// C `questlog_close(co, quest)`.
    QuestClose { player_id: CharacterId, quest: u8 },
    /// Quest 25 ("Earning the Lockpick") completion tail (`two.c:1881-
    /// 1899`): resets all six `thief_killed` counters, sets `thief_bits
    /// |= 1`, marks quest 25 done (nominal `exp: 0` in `QUEST_TABLE` -
    /// the real reward was already granted directly via `World::give_exp`
    /// before this event was queued, matching C's own call order: the
    /// manual `give_exp` happens *before* `questlog_done`), and hands
    /// over a fresh `lockpick`.
    Quest25Reward { player_id: CharacterId },
    /// Quest 26 ("Extortion") completion tail (`two.c:1940-1957`): same
    /// shape as `Quest25Reward` but only resets `thief_killed[0]`, sets
    /// `thief_bits |= 2`, hands over `sewer_key1`.
    Quest26Reward { player_id: CharacterId },
    /// Quest 27 ("Price Fix Exposed") completion (`NT_GIVE`,
    /// `two.c:2124-2139`): real `exp: 15000` reward via the generic
    /// `complete_legacy` path, `thief_bits |= 4`, hands over `sewer_key2`.
    Quest27Done { player_id: CharacterId },
    /// Quest 28 ("The Golden Lockpick") completion (`NT_GIVE`,
    /// `two.c:2140-2154`): `thief_bits |= 8`, hands over `palace_key3`.
    Quest28Done { player_id: CharacterId },
}

impl World {
    /// C `thiefmaster`'s per-tick body (`two.c:1746-2188`).
    pub fn process_two_thiefmaster_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TwoThiefMasterPlayerFacts>,
        realtime: i32,
        area_id: u16,
    ) -> Vec<TwoThiefMasterOutcomeEvent> {
        let thiefmaster_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TWOTHIEFMASTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for thiefmaster_id in thiefmaster_ids {
            self.process_two_thiefmaster_tick(
                thiefmaster_id,
                player_facts,
                realtime,
                area_id,
                &mut events,
            );
        }
        events
    }

    fn process_two_thiefmaster_tick(
        &mut self,
        thiefmaster_id: CharacterId,
        player_facts: &HashMap<CharacterId, TwoThiefMasterPlayerFacts>,
        realtime: i32,
        area_id: u16,
        events: &mut Vec<TwoThiefMasterOutcomeEvent>,
    ) {
        let Some(thiefmaster_name) = self.characters.get(&thiefmaster_id).map(|c| c.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::TwoThiefMaster(mut data)) = self
            .characters
            .get(&thiefmaster_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&thiefmaster_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.two_thiefmaster_handle_char_message(
                    thiefmaster_id,
                    &mut data,
                    message,
                    player_facts,
                    realtime,
                    area_id,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.two_thiefmaster_handle_text_message(
                    thiefmaster_id,
                    &thiefmaster_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.two_thiefmaster_handle_give_message(
                    thiefmaster_id,
                    message,
                    player_facts,
                    events,
                ),
                NT_GOTHIT => self.two_thiefmaster_handle_gothit_message(thiefmaster_id, message),
                _ => {}
            }
        }

        if let Some(thiefmaster) = self.characters.get_mut(&thiefmaster_id) {
            thiefmaster.driver_state = Some(CharacterDriverState::TwoThiefMaster(data));
        }

        // C `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
        // 0)) return;` (`two.c:2169-2172`). No `fight_driver_follow_
        // invisible` call exists in C here - see the module doc comment.
        if let Some(thiefmaster) = self.characters.get(&thiefmaster_id).cloned() {
            let mut seed = self.legacy_random_seed;
            let attacked = self.fight_driver_attack_visible_and_follow(
                thiefmaster_id,
                &thiefmaster,
                area_id,
                FightDriverSuppressions::default(),
                false,
                &mut |below| legacy_random_below_from_seed(&mut seed, below),
            );
            self.legacy_random_seed = seed;
            if attacked {
                return;
            }
        }
        // C `if (spell_self_driver(cn)) return;` (`two.c:2173-2175`).
        if self.spell_self_simple_baddy(thiefmaster_id) {
            return;
        }

        // C `if (talkdir) turn(cn, talkdir);` (`two.c:2177-2179`).
        if let (Some(thiefmaster), Some((tx, ty))) =
            (self.characters.get(&thiefmaster_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(thiefmaster.x), i32::from(thiefmaster.y), tx, ty)
            {
                if let Some(thiefmaster_mut) = self.characters.get_mut(&thiefmaster_id) {
                    let _ = turn(thiefmaster_mut, direction as u8);
                }
            }
        }

        let data = match self
            .characters
            .get(&thiefmaster_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::TwoThiefMaster(data)) => *data,
            _ => return,
        };

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (secure_move_
        // driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret, lastact))
        // return; }` (`two.c:2181-2185`). `tmpx`/`tmpy` reuse `rest_x`/
        // `rest_y`, the same substitution every other stationary NPC in
        // this codebase makes.
        if data.last_talk_tick + TWO_THIEFMASTER_RETURN_TO_POST_TICKS < self.tick.0 {
            let (post_x, post_y) = self
                .characters
                .get(&thiefmaster_id)
                .map(|thiefmaster| (thiefmaster.rest_x, thiefmaster.rest_y))
                .unwrap_or_default();
            if self.secure_move_driver(
                thiefmaster_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            ) {
                return;
            }
        }
        // C `do_idle(cn, TICKS);` (`two.c:2187`) - not modeled, same
        // precedent as every other stationary dialogue-only NPC in this
        // codebase: it has no observable effect in this message-driven
        // architecture.
    }

    /// C `thiefmaster`'s `NT_CHAR` branch (`two.c:1762-2060`).
    #[allow(clippy::too_many_arguments)]
    fn two_thiefmaster_handle_char_message(
        &mut self,
        thiefmaster_id: CharacterId,
        data: &mut TwoThiefMasterDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoThiefMasterPlayerFacts>,
        realtime: i32,
        area_id: u16,
        events: &mut Vec<TwoThiefMasterOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(thiefmaster) = self.characters.get(&thiefmaster_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`two.c:1766-1769`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message;
        // continue; }` (`two.c:1772-1775`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`two.c:1778-1781`).
        if tick < data.last_talk_tick + TWO_THIEFMASTER_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->
        // current_victim != co) continue;` (`two.c:1783-1786`).
        if tick < data.last_talk_tick + TWO_THIEFMASTER_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`two.c:1789-1792`).
        if thiefmaster_id == player_id
            || !char_see_char(&thiefmaster, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`two.c:1795-1798`).
        if char_dist(&thiefmaster, &player) > TWO_THIEFMASTER_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id).copied() else {
            return;
        };

        // C `if (ppd->thief_state < 4) ppd->thief_state = 4;`
        // (`two.c:1804-1806`) - unconditional on `didsay`, every visit.
        let mut state = facts.thief_state;
        if state < 4 {
            state = 4;
            events.push(TwoThiefMasterOutcomeEvent::UpdateThiefState {
                player_id,
                new_state: state,
            });
        }

        let mut didsay = false;
        // C `switch (ppd->thief_state) { ... }` (`two.c:1808-2053`).
        match state {
            4 => {
                self.npc_say(
                    thiefmaster_id,
                    &format!("Ah. A new member. Welcome, {}.", player.name),
                );
                self.push_state(events, player_id, 5);
                didsay = true;
            }
            5 => {
                if self.character_has_item_template(player_id, IID_AREA17_LOCKPICK) {
                    self.push_state(events, player_id, 11);
                } else {
                    self.npc_say(
                        thiefmaster_id,
                        "Now, lets see... What jobs do I have for a young thief who hasn't earned his lockpick yet...",
                    );
                    events.push(TwoThiefMasterOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 25,
                    });
                    events.push(TwoThiefMasterOutcomeEvent::QuestClose {
                        player_id,
                        quest: 26,
                    });
                    events.push(TwoThiefMasterOutcomeEvent::QuestClose {
                        player_id,
                        quest: 27,
                    });
                    events.push(TwoThiefMasterOutcomeEvent::QuestClose {
                        player_id,
                        quest: 28,
                    });
                    self.push_state(events, player_id, 6);
                    didsay = true;
                }
            }
            6 => {
                self.npc_say(
                    thiefmaster_id,
                    &format!(
                        "Ah. This might be just right for thee. Listen, {}. A band of robbers has settled down in an abandoned section of Exkordon. They are committing crimes without our permission. I want thee to go there, and kill as many robbers as thou canst.",
                        player.name
                    ),
                );
                self.push_state(events, player_id, 7);
                didsay = true;
            }
            7 => {
                self.npc_say(
                    thiefmaster_id,
                    "The robbers section is in the eastern part of Exkordon. Thou canst not miss it - just keep going east till you get attacked.",
                );
                self.push_state(events, player_id, 8);
                didsay = true;
            }
            8 => {
                self.npc_say_bytes(
                    thiefmaster_id,
                    &format!(
                        "If thou thinkst thou hast killed enough robbers, come back here and say: {COL_STR_LIGHT_BLUE}I am done{COL_STR_RESET}"
                    ),
                );
                self.push_state(events, player_id, 9);
                didsay = true;
            }
            // `thief_state == 9`: waiting for the robber mission to
            // finish (`two.c:1849-1854`).
            9 => {
                if realtime.saturating_sub(facts.thief_last_seen) > TWO_THIEFMASTER_WAIT_NAG_SECONDS
                {
                    self.npc_say_bytes(
                        thiefmaster_id,
                        &format!(
                            "Well, art thou done, {}? ({COL_STR_LIGHT_BLUE}I am done{COL_STR_RESET})",
                            player.name
                        ),
                    );
                    didsay = true;
                }
            }
            // `thief_state == 10`: grading the robber-killing mission
            // (`two.c:1855-1900`), reached only via `NT_TEXT`'s "i am
            // done" (`case 16`, below).
            10 => {
                let killed = facts.thief_killed;
                let score = killed[0]
                    + killed[1] * 2
                    + killed[2] * 3
                    + killed[3] * 4
                    + killed[4] * 5
                    + killed[5] * 6;
                if score == 0 {
                    self.npc_say(
                        thiefmaster_id,
                        "But thou hast not killed a single robber? I am disappointed.",
                    );
                    self.push_state(events, player_id, 5);
                    didsay = true;
                } else {
                    let val: i64 = if killed[5] > 0 {
                        self.npc_say(
                            thiefmaster_id,
                            &format!("All hail {}, the robber slayer!", player.name),
                        );
                        20000
                    } else if score < 10 {
                        self.npc_say(
                            thiefmaster_id,
                            "Well, thou didst what thine limited abilities allowed.",
                        );
                        5000
                    } else if score < 30 {
                        self.npc_say(thiefmaster_id, &format!("Nicely done, {}.", player.name));
                        10000
                    } else if score < 60 {
                        self.npc_say(thiefmaster_id, &format!("I am impressed, {}.", player.name));
                        15000
                    } else {
                        self.npc_say(
                            thiefmaster_id,
                            &format!("All hail {}, the robber slayer!", player.name),
                        );
                        20000
                    };
                    self.grant_thiefmaster_reward(
                        player_id,
                        player.level,
                        facts.quest25_count,
                        val,
                        area_id,
                    );
                    self.push_state(events, player_id, 11);
                    events.push(TwoThiefMasterOutcomeEvent::Quest25Reward { player_id });
                    didsay = true;
                }
            }
            11 => {
                if !self.character_has_item_template(player_id, IID_AREA17_LOCKPICK) {
                    self.npc_say(
                        thiefmaster_id,
                        "What? Thou hast lost thine lockpick? Here we go again...",
                    );
                    self.push_state(events, player_id, 5);
                    didsay = true;
                } else if self.character_has_item_template(player_id, IID_AREA17_SEWERKEY1) {
                    self.push_state(events, player_id, 15);
                } else {
                    self.npc_say(
                        thiefmaster_id,
                        &format!(
                            "Now that thou hast earned thine lockpick, {}, I want thee to punish a merchant who hast not paid his bills.",
                            player.name
                        ),
                    );
                    events.push(TwoThiefMasterOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 26,
                    });
                    events.push(TwoThiefMasterOutcomeEvent::QuestClose {
                        player_id,
                        quest: 27,
                    });
                    events.push(TwoThiefMasterOutcomeEvent::QuestClose {
                        player_id,
                        quest: 28,
                    });
                    self.push_state(events, player_id, 12);
                    didsay = true;
                }
            }
            12 => {
                self.npc_say(
                    thiefmaster_id,
                    "Next to the governor's palace is a barrel store. It belongs to this merchant. The name does not matter. Just go there, and burn those barrels down.",
                );
                self.push_state(events, player_id, 13);
                didsay = true;
            }
            // `thief_state == 13`: waiting for barrels to burn down
            // (`two.c:1929`).
            13 => {}
            // `thief_state == 14`: grading the burndown mission
            // (`two.c:1930-1957`), reached via `PlayerRuntime::
            // mark_twocity_burndown_kill` (already ported).
            14 => {
                let score = facts.thief_killed[0];
                let val: i64 = if score < 10 {
                    self.npc_say(
                        thiefmaster_id,
                        &format!(
                            "Ah, {}. I've heard about thine efforts burning down those barrels.",
                            player.name
                        ),
                    );
                    5000
                } else {
                    self.npc_say(
                        thiefmaster_id,
                        &format!("Thou made a nice fire, indeed, {}.", player.name),
                    );
                    10000
                };
                self.npc_say(
                    thiefmaster_id,
                    "Here's a key that might come in handy. It opens most of the doors in the sewers.",
                );
                self.grant_thiefmaster_reward(
                    player_id,
                    player.level,
                    facts.quest26_count,
                    val,
                    area_id,
                );
                self.push_state(events, player_id, 15);
                events.push(TwoThiefMasterOutcomeEvent::Quest26Reward { player_id });
                didsay = true;
            }
            15 => {
                if !self.character_has_item_template(player_id, IID_AREA17_LOCKPICK) {
                    self.npc_say(
                        thiefmaster_id,
                        "What? Thou hast lost thine lockpick? Here we go again...",
                    );
                    self.push_state(events, player_id, 5);
                    didsay = true;
                } else if !self.character_has_item_template(player_id, IID_AREA17_SEWERKEY1) {
                    self.npc_say(
                        thiefmaster_id,
                        "What? Thou hast lost the sewer key? Go burn again...",
                    );
                    self.push_state(events, player_id, 11);
                    didsay = true;
                } else if self.character_has_item_template(player_id, IID_AREA17_SEWERKEY2) {
                    self.push_state(events, player_id, 18);
                } else {
                    self.npc_say(
                        thiefmaster_id,
                        &format!(
                            "I have another job for thee, {}. Some of the merchants in Exkordon decided to fix the prices, and I want to know the exact figures. They all signed an agreement, and I want thee to obtain a copy.",
                            player.name
                        ),
                    );
                    events.push(TwoThiefMasterOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 27,
                    });
                    events.push(TwoThiefMasterOutcomeEvent::QuestClose {
                        player_id,
                        quest: 28,
                    });
                    self.push_state(events, player_id, 16);
                    didsay = true;
                }
            }
            16 => {
                self.npc_say(
                    thiefmaster_id,
                    "One of those merchants is Culd. His shop is fairly close to the governors palace. I'd suggest thou try to sneak in at night and search his shop.",
                );
                self.push_state(events, player_id, 17);
                didsay = true;
            }
            // `thief_state == 17`: waiting for the player to deliver the
            // merchant's note (`two.c:1991`).
            17 => {}
            18 => {
                if !self.character_has_item_template(player_id, IID_AREA17_LOCKPICK) {
                    self.npc_say(
                        thiefmaster_id,
                        "What? Thou hast lost thine lockpick? Here we go again...",
                    );
                    self.push_state(events, player_id, 5);
                    didsay = true;
                } else if !self.character_has_item_template(player_id, IID_AREA17_SEWERKEY1) {
                    self.npc_say(
                        thiefmaster_id,
                        "What? Thou hast lost the sewer key? Go burn again...",
                    );
                    self.push_state(events, player_id, 11);
                    didsay = true;
                } else if !self.character_has_item_template(player_id, IID_AREA17_SEWERKEY2) {
                    self.npc_say(
                        thiefmaster_id,
                        "What? Thou hast lost the second sewer key? You're lucky I lost that agreement too...",
                    );
                    self.push_state(events, player_id, 15);
                    didsay = true;
                } else if self.character_has_item_template(player_id, IID_AREA17_PALACEKEY3) {
                    self.push_state(events, player_id, 20);
                } else {
                    self.npc_say(
                        thiefmaster_id,
                        &format!(
                            "One last job for thee, {}. One of my thieves has lost his lockpick in the sewers, close to the Greenling King. This is a special lockpick, quite valuable, so I'd like thee to find it for me.",
                            player.name
                        ),
                    );
                    events.push(TwoThiefMasterOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 28,
                    });
                    self.push_state(events, player_id, 19);
                    didsay = true;
                }
            }
            // `thief_state == 19`: waiting for the player to deliver the
            // golden lockpick (`two.c:2023`).
            19 => {}
            20 => {
                if !self.character_has_item_template(player_id, IID_AREA17_LOCKPICK) {
                    self.npc_say(
                        thiefmaster_id,
                        "What? Thou hast lost thine lockpick? Here we go again...",
                    );
                    self.push_state(events, player_id, 5);
                    didsay = true;
                } else if !self.character_has_item_template(player_id, IID_AREA17_SEWERKEY1) {
                    self.npc_say(
                        thiefmaster_id,
                        "What? Thou hast lost the sewer key? Go burn again...",
                    );
                    self.push_state(events, player_id, 11);
                    didsay = true;
                } else if !self.character_has_item_template(player_id, IID_AREA17_SEWERKEY2) {
                    self.npc_say(
                        thiefmaster_id,
                        "What? Thou hast lost the second sewer key? You're lucky I lost that agreement too...",
                    );
                    self.push_state(events, player_id, 15);
                    didsay = true;
                } else if !self.character_has_item_template(player_id, IID_AREA17_PALACEKEY3) {
                    self.npc_say(
                        thiefmaster_id,
                        &format!("Uh, about that golden lockpick again, {}...", player.name),
                    );
                    self.push_state(events, player_id, 18);
                    didsay = true;
                } else {
                    self.npc_say(
                        thiefmaster_id,
                        "I hope thou art enjoying thine stay here in Exkordon. I do not have any jobs for thee at the moment.",
                    );
                    self.push_state(events, player_id, 21);
                    didsay = true;
                }
            }
            // `thief_state == 21`: all done (`two.c:2052`).
            21 => {}
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; ppd->thief_last_seen = realtime; }`
        // (`two.c:2054-2059`).
        if didsay {
            data.last_talk_tick = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
            events.push(TwoThiefMasterOutcomeEvent::UpdateThiefLastSeen {
                player_id,
                realtime,
            });
        }
    }

    fn push_state(
        &self,
        events: &mut Vec<TwoThiefMasterOutcomeEvent>,
        player_id: CharacterId,
        new_state: i32,
    ) {
        events.push(TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id,
            new_state,
        });
    }

    /// C `tmp = val; val = questlog_scale(questlog_count(co, qnr), val);
    /// give_exp(co, min(level_value(ch[co].level) / 5, val));` (shared
    /// shape of `two.c:1884-1890` and `:1943-1947`).
    fn grant_thiefmaster_reward(
        &mut self,
        player_id: CharacterId,
        level: u32,
        prior_completions: u8,
        base_exp: i64,
        area_id: u16,
    ) {
        let scaled = scale_exp(prior_completions, base_exp);
        let cap = i64::from(level_value(level)) / 5;
        self.give_exp(player_id, scaled.min(cap), u32::from(area_id));
    }

    /// C `thiefmaster`'s `NT_TEXT` branch (`two.c:2064-2115`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::npc::area17::thiefguard`/`sanwyn`/`two_skelly`/
    /// `alchemist`'s text handlers).
    #[allow(clippy::too_many_arguments)]
    fn two_thiefmaster_handle_text_message(
        &mut self,
        thiefmaster_id: CharacterId,
        thiefmaster_name: &str,
        data: &mut TwoThiefMasterDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoThiefMasterPlayerFacts>,
        events: &mut Vec<TwoThiefMasterOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->
        // current_victim) dat->current_victim = 0;` (`two.c:2067-2069`).
        let tick = self.tick.0;
        if tick > data.last_talk_tick + TWO_THIEFMASTER_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
        {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`two.c:2071-2074`).
        if let Some(current_victim) = data.current_victim {
            if current_victim != speaker_id {
                return;
            }
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`two.c:126-144`):
        // ignore our own talk, non-players/player-likes, not-visible.
        if thiefmaster_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(thiefmaster) = self.characters.get(&thiefmaster_id).cloned() else {
            return;
        };
        if !char_see_char(&thiefmaster, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let thief_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.thief_state)
            .unwrap_or(0);

        let mut didsay = false;
        // C's `analyse_text_driver` calls `say(cn, qa[q].answer, ...)`
        // directly (`two.c:206`), same as every sibling driver.
        match analyse_text_qa(text, thiefmaster_name, &speaker.name, TWOCITY_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(thiefmaster_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`two.c:2077-2099`): five independent,
            // mutually-exclusive-by-range `if`s (not an `else if` chain
            // in C either), each resetting `thief_state` back to the
            // start of its own sub-chain.
            TextAnalysisOutcome::Matched(2) => {
                if thief_state <= 9 {
                    data.last_talk_tick = 0;
                    self.push_state(events, speaker_id, 4);
                }
                if (11..=13).contains(&thief_state) {
                    data.last_talk_tick = 0;
                    self.push_state(events, speaker_id, 11);
                }
                if (15..=17).contains(&thief_state) {
                    data.last_talk_tick = 0;
                    self.push_state(events, speaker_id, 15);
                }
                if (18..=19).contains(&thief_state) {
                    data.last_talk_tick = 0;
                    self.push_state(events, speaker_id, 18);
                }
                if (20..=21).contains(&thief_state) {
                    data.last_talk_tick = 0;
                    self.push_state(events, speaker_id, 20);
                }
                didsay = true;
            }
            // C `case 16:` ("i am done") (`two.c:2100-2108`).
            TextAnalysisOutcome::Matched(16) => {
                if thief_state == 9 {
                    self.push_state(events, speaker_id, 10);
                    self.npc_say(thiefmaster_id, "Thou art done? Now, let's see...");
                } else {
                    self.npc_say(thiefmaster_id, "Hu?");
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`two.c:2111-2114`) - note this does *not* touch `dat->
        // last_talk` (except the explicit resets inside the "repeat"
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `thiefmaster`'s `NT_GIVE` branch (`two.c:2117-2160`).
    fn two_thiefmaster_handle_give_message(
        &mut self,
        thiefmaster_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoThiefMasterPlayerFacts>,
        events: &mut Vec<TwoThiefMasterOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get(&thiefmaster_id)
            .and_then(|thiefmaster| thiefmaster.cursor_item)
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            return;
        };
        let thief_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.thief_state)
            .unwrap_or(0);

        if template_id == IID_AREA17_MERCHANTNOTE1 && thief_state == 17 {
            // C `say(cn, "Ah, yes, that is the agreement I wanted. Nice
            // job, %s. Here, this key will open the remaining sewer
            // doors.", ch[co].name); questlog_done(co, 27);
            // destroy_item_byID(co, IID_AREA17_MERCHANTNOTE1);
            // ppd->thief_bits |= 4; in = create_item("sewer_key2"); ...;
            // ppd->thief_state = 18;` (`two.c:2124-2139`).
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_say(
                thiefmaster_id,
                &format!(
                    "Ah, yes, that is the agreement I wanted. Nice job, {giver_name}. Here, this key will open the remaining sewer doors."
                ),
            );
            self.destroy_items_by_template_id(giver_id, IID_AREA17_MERCHANTNOTE1);
            self.push_state(events, giver_id, 18);
            events.push(TwoThiefMasterOutcomeEvent::Quest27Done {
                player_id: giver_id,
            });
        } else if template_id == IID_AREA17_GOLDENLOCKPICK && thief_state == 19 {
            // C `say(cn, "There it is, my golden lockpick, given to me by
            // the guild master in Aston, for extraordinary service. I
            // thank thee, %s!", ch[co].name); questlog_done(co, 28);
            // destroy_item_byID(co, IID_AREA17_GOLDENLOCKPICK);
            // ppd->thief_bits |= 8; in = create_item("palace_key3"); ...;
            // ppd->thief_state = 20;` (`two.c:2140-2154`).
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_say(
                thiefmaster_id,
                &format!(
                    "There it is, my golden lockpick, given to me by the guild master in Aston, for extraordinary service. I thank thee, {giver_name}!"
                ),
            );
            self.destroy_items_by_template_id(giver_id, IID_AREA17_GOLDENLOCKPICK);
            self.push_state(events, giver_id, 20);
            events.push(TwoThiefMasterOutcomeEvent::Quest28Done {
                player_id: giver_id,
            });
        }

        // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;`
        // (`two.c:2156-2158`) - unconditional, whether or not either
        // branch above matched.
        if let Some(thiefmaster) = self.characters.get_mut(&thiefmaster_id) {
            thiefmaster.cursor_item = None;
        }
        self.destroy_item(item_id);
    }

    /// C `standard_message_driver`'s `NT_GOTHIT` case (`drvlib.c:2512-
    /// 2538`) - the only one of its three cases `thiefmaster`'s own
    /// `standard_message_driver(cn, msg, 0, 0)` call (`two.c:2162`)
    /// reaches (`agressive=0`/`helper=0` no-op `NT_CHAR`/`NT_SEEHIT`); no
    /// driver-specific `NT_GOTHIT` handler exists in C for this NPC - see
    /// the module doc comment.
    fn two_thiefmaster_handle_gothit_message(
        &mut self,
        thiefmaster_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        // C `fight_driver_note_hit(cn);` (`drvlib.c:2514`).
        let tick = self.tick.0 as i32;
        if let Some(thiefmaster) = self.characters.get_mut(&thiefmaster_id) {
            thiefmaster
                .fight_driver
                .get_or_insert_with(FightDriverData::default)
                .last_hit = tick;
        }

        let attacker_id = CharacterId(message.dat1.max(0) as u32);
        // C `co = msg->dat1; if (!co) break;` (`drvlib.c:2516-2519`).
        if attacker_id.0 == 0 {
            return;
        }
        let Some(thiefmaster) = self.characters.get(&thiefmaster_id).cloned() else {
            return;
        };
        let Some(attacker) = self.characters.get(&attacker_id).cloned() else {
            return;
        };
        // C `if (ch[cn].group == ch[co].group) break;` (`drvlib.c:2523-
        // 2525`).
        if thiefmaster.group == attacker.group {
            return;
        }
        // C `if (!can_attack(cn, co)) break;` (`drvlib.c:2526-2528`).
        if !can_attack(&thiefmaster, &attacker, &self.map) {
            return;
        }
        // C `fight_driver_add_enemy(cn, co, 1, ...)` (`drvlib.c:2533-
        // 2536`) - the `visible` argument has no equivalent in
        // `add_simple_baddy_enemy_unchecked`, see the module doc comment.
        if let Some(thiefmaster_mut) = self.characters.get_mut(&thiefmaster_id) {
            let _ = add_simple_baddy_enemy_unchecked(thiefmaster_mut, attacker_id, 1, tick);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;
