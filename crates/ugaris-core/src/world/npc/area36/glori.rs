//! Glori NPC (`CDR_CALIGARGLORI`), "First in charge" of the Caligar
//! library, who runs the quest-54-58 chain: report the training-facility
//! obelisks (quest 55), investigate the dungeon and bring back the three
//! obelisks (quest 56), retrieve the three palace key parts (quest 57),
//! and forge/deliver the underground key (quest 58).
//!
//! Ports `src/area/36/caligar.c::glori_driver` (`:519-803`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:86-212`, ported as
//! [`super::AREA36_QA`] in `world::npc::area36`).
//!
//! Unlike every `warrmines.c` dialogue NPC (`world::npc::area31::*`),
//! `glori_driver` keeps **no NPC-local state at all** - no `set_data(cn,
//! ...)` call for itself, only `set_data(co, DRD_CALIGAR_PPD, ...)` for the
//! *player* (`ppd->glori_state`/`glori_last_talk`). The "pause facing the
//! speaker, then resume patrol" behavior instead reuses `ch[cn].
//! clan_serial` (`crate::entity::Character::clan_serial`) as a raw 10-tick
//! countdown - the same field `world::npc::clubmaster`/`area30::clanmaster`
//! use for actual clan membership, just borrowed here as a scratch
//! counter, matching C exactly (same struct field, unrelated purpose).
//! Ported by mutating `Character::clan_serial` directly since it lives on
//! `World`, no outcome event needed.
//!
//! Glori's 19-state (`0`-`18`) chain has three fallthrough points where a
//! `has_item`-gated state advances twice in the same tick, only speaking
//! the second (landing) state's line - collapsed into single match arms
//! below, same precedent as `world::npc::area31::dwarfchief`'s own
//! quest-transition fallthroughs:
//! - `case 10` (all three obelisks) falls into `case 11`'s "Wow, these are
//!   most interesting..." line, landing on state `12`.
//! - `case 12` (all three key parts) falls into `case 13`'s "Great job..."
//!   line, landing on state `14`.
//! - `case 16` (the assembled dungeon key) falls into `case 17`'s "Well
//!   done, %s..." line, landing on state `18`.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `realtime` (wall-clock seconds) drives `glori_last_talk`, not
//!   `ticker` - `now: i32` is threaded in the same way as
//!   `world::npc::area36::caligar_guard`'s own `now` parameter.
//! - The `NT_GIVE` handler never actually keeps or consumes an obelisk -
//!   `has_item` gates on state `10` are the only thing that matters, so
//!   every obelisk handed over is always given straight back (with a hint
//!   line if the other two are still missing), matching C exactly.
//! - C's per-message `remove_message(cn, msg)` calls have no equivalent
//!   here - the per-tick `driver_messages` drain (`std::mem::take`)
//!   already empties the queue exactly once per tick, same precedent as
//!   every other ported NPC driver.
//! - C's unconditional `do_idle(cn, TICKS)` tail call is not reachable in
//!   this driver (the `secure_move_driver`/`clan_serial` branch always
//!   returns or falls through to the `turn` call), matching the
//!   established precedent of not porting trailing unconditional
//!   `do_idle` calls.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::item_driver::{
    IID_CALIGARDUNGEONKEY, IID_CALIGARKEYP1, IID_CALIGARKEYP2, IID_CALIGARKEYP3,
    IID_CALIGAROBELISK1, IID_CALIGAROBELISK2, IID_CALIGAROBELISK3,
};
use crate::world::*;

use super::AREA36_QA;

/// C `char_dist(cn, co) > 10` (`caligar.c:551`/`:754`): both `NT_CHAR` and
/// `NT_TEXT` use the same threshold.
const CALIGAR_GLORI_DISTANCE: i32 = 10;
/// C `realtime - ppd->glori_last_talk < 4` (`caligar.c:562`).
const CALIGAR_GLORI_TALK_COOLDOWN_SECONDS: i32 = 4;
/// C `ch[cn].clan_serial = 10` (`caligar.c:712`).
const CALIGAR_GLORI_TALK_STALL_TICKS: u32 = 10;

/// Per-player facts [`World::process_caligar_glori_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. `has_item`
/// checks (obelisks/key parts/dungeon key) are resolved directly via
/// `World::character_has_item_template` instead, since items live on
/// `World`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaligarGloriPlayerFacts {
    /// `PlayerRuntime::caligar_glori_state()`.
    pub glori_state: i32,
    /// `PlayerRuntime::caligar_glori_last_talk()`.
    pub glori_last_talk: i32,
    /// `PlayerRuntime::caligar_watch_flag()`.
    pub watch_flag: i32,
}

/// A side effect [`World::process_caligar_glori_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaligarGloriOutcomeEvent {
    /// C `ppd->glori_state++; ppd->glori_last_talk = realtime;`
    /// (every successful `glori_driver` state transition).
    AdvanceGloriTalk {
        player_id: CharacterId,
        new_state: i32,
        realtime_seconds: i32,
    },
    /// C `questlog_open(co, N)`.
    QuestOpen { player_id: CharacterId, quest: u32 },
    /// C `questlog_done(co, N)`.
    QuestDone { player_id: CharacterId, quest: u32 },
    /// C `case 2:` (`analyse_text_driver` code `2`, "repeat"/"restart"):
    /// resets back to the start of whichever mini-block is in progress
    /// (`caligar.c:759-782`).
    ResetGloriMiniBlock { player_id: CharacterId },
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_CALIGARGLORI`
    /// characters (C `ch_driver`'s `CDR_CALIGARGLORI` case,
    /// `caligar.c:1863-1865`).
    pub fn process_caligar_glori_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CaligarGloriPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<CaligarGloriOutcomeEvent> {
        let glori_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_CALIGARGLORI
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for glori_id in glori_ids {
            self.process_caligar_glori_messages(glori_id, player_facts, now, &mut events);
            self.caligar_glori_stall_or_move(glori_id, area_id);
        }
        events
    }

    fn process_caligar_glori_messages(
        &mut self,
        glori_id: CharacterId,
        player_facts: &HashMap<CharacterId, CaligarGloriPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarGloriOutcomeEvent>,
    ) {
        let Some(glori_name) = self
            .characters
            .get(&glori_id)
            .map(|glori| glori.name.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&glori_id)
            .map(|glori| std::mem::take(&mut glori.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.caligar_glori_handle_char_message(
                    glori_id,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => {
                    self.caligar_glori_handle_text_message(glori_id, &glori_name, message, events)
                }
                NT_GIVE => self.caligar_glori_handle_give_message(glori_id, message),
                _ => {}
            }
        }

        // C `if (talkdir) turn(cn, talkdir);` (`caligar.c:798-800`).
        if let (Some(glori), Some((tx, ty))) =
            (self.characters.get(&glori_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(glori.x), i32::from(glori.y), tx, ty) {
                if let Some(glori_mut) = self.characters.get_mut(&glori_id) {
                    let _ = turn(glori_mut, direction as u8);
                }
            }
        }
    }

    /// C `if (ch[cn].clan_serial > 0) ch[cn].clan_serial--; else if
    /// (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
    /// lastact)) return;` (`caligar.c:792-796`).
    fn caligar_glori_stall_or_move(&mut self, glori_id: CharacterId, area_id: u16) {
        let Some(glori) = self.characters.get(&glori_id) else {
            return;
        };
        if glori.clan_serial > 0 {
            if let Some(glori_mut) = self.characters.get_mut(&glori_id) {
                glori_mut.clan_serial -= 1;
            }
            return;
        }
        let (post_x, post_y) = (glori.rest_x, glori.rest_y);
        self.secure_move_driver(
            glori_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        );
    }

    /// C `glori_driver`'s `NT_CHAR` branch (`caligar.c:529-714`).
    #[allow(clippy::too_many_arguments)]
    fn caligar_glori_handle_char_message(
        &mut self,
        glori_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaligarGloriPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarGloriOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(glori) = self.characters.get(&glori_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if player.driver == CDR_LOSTCON {
            return;
        }
        if glori_id == player_id || !char_see_char(&glori, &player, &self.map, self.date.daylight) {
            return;
        }
        if char_dist(&glori, &player) > CALIGAR_GLORI_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        if now - facts.glori_last_talk < CALIGAR_GLORI_TALK_COOLDOWN_SECONDS {
            return;
        }

        let has_obelisks = self.character_has_item_template(player_id, IID_CALIGAROBELISK1)
            && self.character_has_item_template(player_id, IID_CALIGAROBELISK2)
            && self.character_has_item_template(player_id, IID_CALIGAROBELISK3);
        let has_key_parts = self.character_has_item_template(player_id, IID_CALIGARKEYP1)
            && self.character_has_item_template(player_id, IID_CALIGARKEYP2)
            && self.character_has_item_template(player_id, IID_CALIGARKEYP3);
        let has_dungeon_key = self.character_has_item_template(player_id, IID_CALIGARDUNGEONKEY);

        // C `switch (ppd->glori_state) { ... }` (`caligar.c:567-709`). The
        // `case 10`/`case 12`/`case 16` fallthroughs are collapsed into
        // their landing state's line - see the module doc comment.
        // `Quest` records which `questlog_done`/`questlog_open` calls that
        // state's transition makes: `Done(q)` only, or `Done(q1)` followed
        // by `Open(q2)`.
        enum Quest {
            None,
            Done(u32),
            DoneThenOpen(u32, u32),
        }
        let outcome: Option<(&str, i32, Quest)> = match facts.glori_state {
            0 => Some(("Thank you for coming %s!", 1, Quest::DoneThenOpen(54, 55))),
            1 => Some(("I am Glori, First in charge.", 2, Quest::None)),
            2 => Some((
                "We must find out what those mages intend to do with that plaque and retrieve it as soon as possible.",
                3,
                Quest::None,
            )),
            3 => Some((
                "We are currently working in secrecy with the guard outside of this library. He has informed me that the mages have set up three training facilities to train their minions.",
                4,
                Quest::None,
            )),
            4 => Some((
                "Travel to the three training facilities to the east and examine the minions fighting styles. Come back to me with your findings.",
                5,
                Quest::None,
            )),
            5 if facts.watch_flag >= (1 | 2 | 4) => Some(("Hello, %s.", 6, Quest::None)),
            6 => Some(("__GLORI_TELL_FINDINGS__", 7, Quest::None)),
            7 => Some((
                "Ah, good work. Now that we know how the enemies fight, we can prepare ourselves for battle.",
                8,
                Quest::DoneThenOpen(55, 56),
            )),
            8 => Some((
                "I have gotten a report that there is a dungeon below one of the traders' shops. It is said to lead to a dungeon full of zombies, and I assume there will be two more like it containing skeletons and vampires.",
                9,
                Quest::None,
            )),
            9 => Some((
                "Please go and investigate this and report back with your findings, if any.",
                10,
                Quest::None,
            )),
            10 if has_obelisks => Some((
                "Wow, these are most interesting. I suggest you speak with the guard outside and ask if he knows of anyone that may be able to tell you what these are for.",
                12,
                Quest::DoneThenOpen(56, 57),
            )),
            11 => Some((
                "Wow, these are most interesting. I suggest you speak with the guard outside and ask if he knows of anyone that may be able to tell you what these are for.",
                12,
                Quest::None,
            )),
            12 if has_key_parts => Some((
                "Great job. I feel we are very close to putting an end to those evil mages ways.",
                14,
                Quest::DoneThenOpen(57, 58),
            )),
            13 => Some((
                "Great job. I feel we are very close to putting an end to those evil mages ways.",
                14,
                Quest::None,
            )),
            14 => Some((
                "Hmm. It might be a good idea to take the key parts to the blacksmith south west of this library, near the bar. He may be able to forge these together.",
                15,
                Quest::None,
            )),
            15 => Some((
                "If he can make a complete key from these, he'll probably need some sort of payment. Once the key is made, take it Arquin out front. He should be able to tell you where to go with it.",
                16,
                Quest::None,
            )),
            16 if has_dungeon_key => Some(("__GLORI_WELL_DONE__", 18, Quest::Done(58))),
            17 => Some(("__GLORI_WELL_DONE__", 18, Quest::None)),
            _ => None,
        };

        let Some((line, new_state, quest_transition)) = outcome else {
            return;
        };

        match quest_transition {
            Quest::None => {}
            Quest::Done(quest) => {
                events.push(CaligarGloriOutcomeEvent::QuestDone { player_id, quest });
            }
            Quest::DoneThenOpen(done_quest, open_quest) => {
                events.push(CaligarGloriOutcomeEvent::QuestDone {
                    player_id,
                    quest: done_quest,
                });
                events.push(CaligarGloriOutcomeEvent::QuestOpen {
                    player_id,
                    quest: open_quest,
                });
            }
        }

        // C `case 6:` uses `log_char`, not `quiet_say` (`caligar.c:614`);
        // C `case 17:` substitutes the player's name (`caligar.c:702`).
        match line {
            "__GLORI_TELL_FINDINGS__" => {
                self.queue_system_text(player_id, "You tell Glori what you have seen.");
            }
            "__GLORI_WELL_DONE__" => {
                self.npc_quiet_say(
                    glori_id,
                    &format!("Well done, {}. Did you talk to Homden yet?", player.name),
                );
            }
            _ => {
                self.npc_quiet_say(glori_id, &fill_player_name(line, &player.name));
            }
        }

        events.push(CaligarGloriOutcomeEvent::AdvanceGloriTalk {
            player_id,
            new_state,
            realtime_seconds: now,
        });

        *face_target = Some((i32::from(player.x), i32::from(player.y)));
        if let Some(glori_mut) = self.characters.get_mut(&glori_id) {
            glori_mut.clan_serial = CALIGAR_GLORI_TALK_STALL_TICKS;
        }
    }

    /// C `glori_driver`'s `NT_TEXT` branch (`caligar.c:743-784`), wired
    /// through the generic `analyse_text_qa` matcher.
    fn caligar_glori_handle_text_message(
        &mut self,
        glori_id: CharacterId,
        glori_name: &str,
        message: &CharacterDriverMessage,
        events: &mut Vec<CaligarGloriOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(glori) = self.characters.get(&glori_id).cloned() else {
            return;
        };
        if glori_id == speaker_id || !char_see_char(&glori, &speaker, &self.map, self.date.daylight)
        {
            return;
        }
        if char_dist(&glori, &speaker) > CALIGAR_GLORI_DISTANCE {
            return;
        }

        match analyse_text_qa(text, glori_name, &speaker.name, AREA36_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(glori_id, &reply);
            }
            TextAnalysisOutcome::Matched(2) => {
                events.push(CaligarGloriOutcomeEvent::ResetGloriMiniBlock {
                    player_id: speaker_id,
                });
            }
            TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
        }
    }

    /// C `glori_driver`'s `NT_GIVE` branch (`caligar.c:717-740`).
    fn caligar_glori_handle_give_message(
        &mut self,
        glori_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&glori_id)
            .and_then(|glori| glori.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
            return;
        };

        // C `if (it[in].ID == IID_CALIGAROBELISK1/2/3) { ... hint ... }`
        // (`caligar.c:720-734`).
        if item.template_id == IID_CALIGAROBELISK1
            || item.template_id == IID_CALIGAROBELISK2
            || item.template_id == IID_CALIGAROBELISK3
        {
            let has3 = self.character_has_item_template(giver_id, IID_CALIGAROBELISK3);
            let has1 = self.character_has_item_template(giver_id, IID_CALIGAROBELISK1);
            let has2 = self.character_has_item_template(giver_id, IID_CALIGAROBELISK2);
            if !has3 && item.template_id != IID_CALIGAROBELISK3 {
                self.npc_quiet_say(
                    glori_id,
                    "You will need all three of them. I've heard a heavy drinker tell about another dungeon being hidden in his favorite place.",
                );
            } else if !has1 && item.template_id != IID_CALIGAROBELISK1 {
                self.npc_quiet_say(
                    glori_id,
                    "You will need all three of them. One of them, so rumor has it, is behind a large building.",
                );
            } else if !has2 && item.template_id != IID_CALIGAROBELISK2 {
                self.npc_quiet_say(
                    glori_id,
                    "You will need all three of them. As I said before, one should be accessible through a shop.",
                );
            }
        }

        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

/// C `quiet_say(cn, qa[q].answer, ...)`'s `%s` substitution, applied here
/// for `glori_driver`'s own plain `quiet_say(cn, "...%s!", ch[co].name)`
/// lines (`case 0`).
fn fill_player_name(template: &str, player_name: &str) -> String {
    template.replacen("%s", player_name, 1)
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_CALIGARGLORI;
